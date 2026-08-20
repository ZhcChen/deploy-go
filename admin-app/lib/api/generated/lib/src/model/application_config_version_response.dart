//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'application_config_version_response.g.dart';

/// ApplicationConfigVersionResponse
///
/// Properties:
/// * [applicationConfigFileId]
/// * [configVersion]
/// * [createdAt]
/// * [createdBy]
/// * [digest]
/// * [id]
/// * [source_]
/// * [sourceTemplateDigest]
/// * [sourceVersionId]
@BuiltValue()
abstract class ApplicationConfigVersionResponse implements Built<ApplicationConfigVersionResponse, ApplicationConfigVersionResponseBuilder> {
  @BuiltValueField(wireName: r'application_config_file_id')
  String get applicationConfigFileId;

  @BuiltValueField(wireName: r'config_version')
  int get configVersion;

  @BuiltValueField(wireName: r'created_at')
  String get createdAt;

  @BuiltValueField(wireName: r'created_by')
  String? get createdBy;

  @BuiltValueField(wireName: r'digest')
  String? get digest;

  @BuiltValueField(wireName: r'id')
  String get id;

  @BuiltValueField(wireName: r'source')
  String get source_;

  @BuiltValueField(wireName: r'source_template_digest')
  String? get sourceTemplateDigest;

  @BuiltValueField(wireName: r'source_version_id')
  String? get sourceVersionId;

  ApplicationConfigVersionResponse._();

  factory ApplicationConfigVersionResponse([void updates(ApplicationConfigVersionResponseBuilder b)]) = _$ApplicationConfigVersionResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ApplicationConfigVersionResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ApplicationConfigVersionResponse> get serializer => _$ApplicationConfigVersionResponseSerializer();
}

class _$ApplicationConfigVersionResponseSerializer implements PrimitiveSerializer<ApplicationConfigVersionResponse> {
  @override
  final Iterable<Type> types = const [ApplicationConfigVersionResponse, _$ApplicationConfigVersionResponse];

  @override
  final String wireName = r'ApplicationConfigVersionResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ApplicationConfigVersionResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'application_config_file_id';
    yield serializers.serialize(
      object.applicationConfigFileId,
      specifiedType: const FullType(String),
    );
    yield r'config_version';
    yield serializers.serialize(
      object.configVersion,
      specifiedType: const FullType(int),
    );
    yield r'created_at';
    yield serializers.serialize(
      object.createdAt,
      specifiedType: const FullType(String),
    );
    if (object.createdBy != null) {
      yield r'created_by';
      yield serializers.serialize(
        object.createdBy,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.digest != null) {
      yield r'digest';
      yield serializers.serialize(
        object.digest,
        specifiedType: const FullType.nullable(String),
      );
    }
    yield r'id';
    yield serializers.serialize(
      object.id,
      specifiedType: const FullType(String),
    );
    yield r'source';
    yield serializers.serialize(
      object.source_,
      specifiedType: const FullType(String),
    );
    if (object.sourceTemplateDigest != null) {
      yield r'source_template_digest';
      yield serializers.serialize(
        object.sourceTemplateDigest,
        specifiedType: const FullType.nullable(String),
      );
    }
    if (object.sourceVersionId != null) {
      yield r'source_version_id';
      yield serializers.serialize(
        object.sourceVersionId,
        specifiedType: const FullType.nullable(String),
      );
    }
  }

  @override
  Object serialize(
    Serializers serializers,
    ApplicationConfigVersionResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required ApplicationConfigVersionResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'application_config_file_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.applicationConfigFileId = valueDes;
          break;
        case r'config_version':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.configVersion = valueDes;
          break;
        case r'created_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.createdAt = valueDes;
          break;
        case r'created_by':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.createdBy = valueDes;
          break;
        case r'digest':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.digest = valueDes;
          break;
        case r'id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.id = valueDes;
          break;
        case r'source':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.source_ = valueDes;
          break;
        case r'source_template_digest':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.sourceTemplateDigest = valueDes;
          break;
        case r'source_version_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType.nullable(String),
          ) as String?;
          if (valueDes == null) continue;
          result.sourceVersionId = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  ApplicationConfigVersionResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ApplicationConfigVersionResponseBuilder();
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
