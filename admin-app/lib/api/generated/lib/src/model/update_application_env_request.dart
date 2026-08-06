//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'update_application_env_request.g.dart';

/// UpdateApplicationEnvRequest
///
/// Properties:
/// * [content]
/// * [expectedVersion]
@BuiltValue()
abstract class UpdateApplicationEnvRequest implements Built<UpdateApplicationEnvRequest, UpdateApplicationEnvRequestBuilder> {
  @BuiltValueField(wireName: r'content')
  String get content;

  @BuiltValueField(wireName: r'expected_version')
  int get expectedVersion;

  UpdateApplicationEnvRequest._();

  factory UpdateApplicationEnvRequest([void updates(UpdateApplicationEnvRequestBuilder b)]) = _$UpdateApplicationEnvRequest;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(UpdateApplicationEnvRequestBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<UpdateApplicationEnvRequest> get serializer => _$UpdateApplicationEnvRequestSerializer();
}

class _$UpdateApplicationEnvRequestSerializer implements PrimitiveSerializer<UpdateApplicationEnvRequest> {
  @override
  final Iterable<Type> types = const [UpdateApplicationEnvRequest, _$UpdateApplicationEnvRequest];

  @override
  final String wireName = r'UpdateApplicationEnvRequest';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    UpdateApplicationEnvRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'content';
    yield serializers.serialize(
      object.content,
      specifiedType: const FullType(String),
    );
    yield r'expected_version';
    yield serializers.serialize(
      object.expectedVersion,
      specifiedType: const FullType(int),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    UpdateApplicationEnvRequest object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required UpdateApplicationEnvRequestBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'content':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.content = valueDes;
          break;
        case r'expected_version':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.expectedVersion = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  UpdateApplicationEnvRequest deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = UpdateApplicationEnvRequestBuilder();
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
