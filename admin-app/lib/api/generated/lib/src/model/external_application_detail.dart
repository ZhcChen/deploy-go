//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:deploy_go_api_client/src/model/external_deployment_target.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'external_application_detail.g.dart';

/// ExternalApplicationDetail
///
/// Properties:
/// * [description]
/// * [id]
/// * [name]
/// * [slug]
/// * [status]
/// * [targets]
@BuiltValue()
abstract class ExternalApplicationDetail implements Built<ExternalApplicationDetail, ExternalApplicationDetailBuilder> {
  @BuiltValueField(wireName: r'description')
  String get description;

  @BuiltValueField(wireName: r'id')
  String get id;

  @BuiltValueField(wireName: r'name')
  String get name;

  @BuiltValueField(wireName: r'slug')
  String get slug;

  @BuiltValueField(wireName: r'status')
  String get status;

  @BuiltValueField(wireName: r'targets')
  BuiltList<ExternalDeploymentTarget> get targets;

  ExternalApplicationDetail._();

  factory ExternalApplicationDetail([void updates(ExternalApplicationDetailBuilder b)]) = _$ExternalApplicationDetail;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ExternalApplicationDetailBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ExternalApplicationDetail> get serializer => _$ExternalApplicationDetailSerializer();
}

class _$ExternalApplicationDetailSerializer implements PrimitiveSerializer<ExternalApplicationDetail> {
  @override
  final Iterable<Type> types = const [ExternalApplicationDetail, _$ExternalApplicationDetail];

  @override
  final String wireName = r'ExternalApplicationDetail';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ExternalApplicationDetail object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'description';
    yield serializers.serialize(
      object.description,
      specifiedType: const FullType(String),
    );
    yield r'id';
    yield serializers.serialize(
      object.id,
      specifiedType: const FullType(String),
    );
    yield r'name';
    yield serializers.serialize(
      object.name,
      specifiedType: const FullType(String),
    );
    yield r'slug';
    yield serializers.serialize(
      object.slug,
      specifiedType: const FullType(String),
    );
    yield r'status';
    yield serializers.serialize(
      object.status,
      specifiedType: const FullType(String),
    );
    yield r'targets';
    yield serializers.serialize(
      object.targets,
      specifiedType: const FullType(BuiltList, [FullType(ExternalDeploymentTarget)]),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    ExternalApplicationDetail object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required ExternalApplicationDetailBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'description':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.description = valueDes;
          break;
        case r'id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.id = valueDes;
          break;
        case r'name':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.name = valueDes;
          break;
        case r'slug':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.slug = valueDes;
          break;
        case r'status':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.status = valueDes;
          break;
        case r'targets':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(ExternalDeploymentTarget)]),
          ) as BuiltList<ExternalDeploymentTarget>;
          result.targets.replace(valueDes);
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  ExternalApplicationDetail deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ExternalApplicationDetailBuilder();
    final serializedList = (serialized as Iterable<Object?>).toList();
    final unhandled = <Object?>[];
    _deserializeProperties(
      serializers,
      serialized,
      specifiedType: specifiedType,
      serializedList: serializedList,
      unhandled: unhandled,
      result: result,
    );
    return result.build();
  }
}
